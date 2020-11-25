from .handler_decorator import *

import jedi
from uuid import uuid4
from collections import OrderedDict
import re

SPHINX = re.compile(r"\s*:param\s+(?P<param>\w+):\s*(?P<doc>[^\n]+)")
EPYDOC = re.compile(r"\s*@param\s+(?P<param>\w+):\s*(?P<doc>[^\n]+)")
GOOGLE = re.compile(r"\s*[*]{0,2}(?P<param>\w+).*:\s*(?P<doc>[^\n]+)")

DOC_REGEX = [SPHINX, EPYDOC, GOOGLE]

def _param_docs(docstring, param_name):
    for line in docstring.splitlines():
        for regex in DOC_REGEX:
            m = regex.match(line)
            if not m:
                continue
            if m.group('param') != param_name:
                continue
            return m.group('doc') or ""

def format_docstring(contents):
    """Python doc strings come in a number of formats, but LSP wants markdown.
    Until we can find a fast enough way of discovering and parsing each format,
    we can do a little better by at least preserving indentation.
    """
    if contents is None:
        return contents
    contents = contents.replace('\t', u'\u00A0' * 4)
    contents = contents.replace('  ', u'\u00A0' * 2)
    return contents


class LRU(OrderedDict):
    'Limit size, evicting the least recently looked-up key when full'

    def __init__(self, maxsize=5, *args, **kwdargs):
        self.maxsize = maxsize
        super().__init__(*args, **kwdargs)

    def __getitem__(self, key):
        value = super().__getitem__(key)
        self.move_to_end(key)
        return value

    def __setitem__(self, key, value):
        if key in self:
            self.move_to_end(key)
        super().__setitem__(key, value)
        if len(self) > self.maxsize:
            oldest = next(iter(self))
            del self[oldest]

@collect_handlers("message_handlers")
class Completer:
    def __init__(self, executor, *, uuid):
        self.executor = executor
        self.uuid = uuid
        self.code = None
        self.states = LRU()

    async def handle_message_a(self, subcmd, **kwargs):
        if subcmd not in self.message_handlers:
            raise Exception(f'Message with unrecognized subcommand "{subcmd}"')
        handler = self.message_handlers[subcmd]
        await handler(self, **kwargs)

    async def send_message_a(self, subcmd, subuuid, **kwargs):
        await self.executor.send_message_a("complete", self.uuid, subcmd=subcmd, subuuid=subuuid, **kwargs)
    
    @handle("signatures")
    async def get_signature_help_a(self, subuuid, code, lineNumber, column):
        try:

            interpreter = jedi.Interpreter(code, [self.executor.namespace])
            jedi_signatures =  interpreter.get_signatures(line=lineNumber, column=column)
            # For some reason, get_type_hint doesn't work the same on signatures as on completions...
            [signatures, full_name, root] = self.get_signature_help_helper(jedi_signatures, code)
            await self.send_message_a("signatures", subuuid, signatures=signatures, full_name=full_name, root=root)
        except KeyboardInterrupt:
            pass
    
    def get_signature_help_helper(self, jedi_signatures, code):
        import jedi
        if not jedi_signatures:
            return [None, None, None]

        s = jedi_signatures[0]
        # docstring() returns a signature with fully qualified type names.
        # This is ugly. get_type_hint() does better but it only works on Completion objects,
        # not on Signature. Thus, we get a completion object. To do so, we ask for a completion at
        # the open bracket of the current function.
        completion = jedi.Interpreter(code, [self.executor.namespace]).complete(*s.bracket_start)[0]
        try:
            function_sig = completion.get_type_hint()
        except NotImplementedError:
            return [None, None, None]
            
        [full_name, root] = self.get_fullname_root(completion)
        if function_sig and completion.parent().type == "instance":
            function_sig = function_sig.replace("self, ", "")
        sig = {
            'label': function_sig,
            'documentation': format_docstring(s.docstring(raw=True))
        }

        # If there are params, add those
        if s.params:
            sig['parameters'] = [{
                'label': p.name,
                'documentation': _param_docs(s.docstring(), p.name)
            } for p in s.params]

        # We only return a single signature because Python doesn't allow overloading
        sig_info = {'signatures': [sig], 'activeSignature': 0}

        if s.index is not None and s.params:
            # Then we know which parameter we're looking at
            sig_info['activeParameter'] = s.index
        return [sig_info, full_name, root]

    @handle("completions")
    async def get_completions_a(self, subuuid, code, lineNumber, column):
        try:
            self.code = code
            state_id = str(uuid4())
            completions = jedi.Interpreter(code, [self.executor.namespace]) \
                            .complete(line=lineNumber, column=column, fuzzy=True)
            self.states[state_id] = completions
            result = []
            for comp in completions:
                result.append(dict(
                    name=comp.name, 
                    kind=comp.type
                ))
            await self.send_message_a("completions", subuuid, completions=result, state_id=state_id)
        except KeyboardInterrupt:
                pass 

    @handle("completion_detail")
    async def get_completion_info_a(self, subuuid, state_id, idx):
        completion = self.states[state_id][idx]
        try:
            # Try getting name and root for link to api docs.
            # Will fail on properties.
            [full_name, root] = self.get_fullname_root(completion)
            if completion.type == "instance":
                [docstring, signature, full_name, root] = self.get_completion_info_instance(subuuid, completion)
            elif completion.type in ["function", "method"]:
                [docstring, signature] = self.get_completion_info_function_or_method(subuuid, completion)
            elif completion.type == "module":
                signature = completion.infer()[0].full_name
                docstring = completion.docstring(raw=True)
            else:
                signature = completion._get_docstring_signature()
                docstring = completion.docstring(raw=True)
        except Exception as e:
            print("Error triggered during completion detail for", completion.name, "type:", completion.type)
            raise
        except KeyboardInterrupt:
            return

        # import re
        # regex = re.compile('(?<!\n)\n(?!\n)', re.MULTILINE) # Remove isolated newline characters.
        # docstring = regex.sub("", docstring)
        await self.send_message_a("completion_detail", subuuid, docstring=format_docstring(docstring), signature=signature, full_name=full_name, root=root)

    def get_fullname_root(self, completion):
        if completion.name.startswith("_") or completion.name in ["from_json", "to_json"]:
            return [None, None]
        try:
            full_name = completion.infer()[0].full_name
        except IndexError:
            return [None, None]
        if not full_name or not full_name.startswith("spectralsequence_chart"):
            return [None, None]
        if completion.type in ["class", "module"]:
            return [full_name, full_name]
        root = ".".join(full_name.split(".")[:-1])
        return [full_name, root]

    
    def get_completion_info_instance(self, subuuid, completion):
        """ Jedi by default does a bad job of getting the completion info for "instances". 
            If the instance is a property on a class with an available docstring, then we report that.
            In any case, give the signature as "name: type".
        """
        docstring = ""
        type_string = ""
        full_name = None
        root = None
        try:
            # Jedi makes it a bit tricky to get from the Jedi wrapper object to the object it refers to...
            object = completion.get_signatures()[0]._name._value.access_handle.access._obj
            parent_object = completion._name._wrapped_name._parent_value.access_handle.access._obj
            parent_type = type(parent_object)
            object = None
            from inspect import getdoc    
            if hasattr(parent_type, completion.name):
                prop = getattr(parent_type, completion.name)
                docstring = getdoc(prop)
                object = prop.fget
            elif type(getattr(parent_object, completion.name)) is property:
                prop = getattr(parent_object, completion.name)
                docstring = getdoc(prop)
                object = prop.fget
            if object.__module__.startswith("spectralsequence_chart"):
                full_name = f"{object.__module__}.{object.__qualname__}"
                root = ".".join(full_name.split(".")[:-1])
            # full_name = object.full_name
            if object:
                from parso import parse
                from inspect import getsource                
                # In this case, type(object).__name__ unfortunately gives "property", which isn't very descriptive.
                # We would like to get the actual type, so we use parso to extract the type from the source.
                # This will throw OSError for interpreter defined classes, but we don't expect many of those.
                funcdef = next(parse(getsource(object)).iter_funcdefs()) 
                type_string = funcdef.annotation.get_code()

        except (AttributeError, OSError): # AttributeError:
            pass
        if type_string:
            signature = f"{completion.name}: {type_string}"
        else:
            signature = ""
        return [docstring, signature, full_name, root]

    def get_completion_info_function_or_method(self, subuuid, completion):
        docstring = completion.docstring(raw=True) or completion._get_docstring()
        try:
            # Collect the return type signature for the method. TODO: this only should be used for type function or method.
            # docstring() returns a signature with fully qualified type names.
            # This is ugly, so we use get_type_hint() instead.
            signature = completion.get_type_hint()
            if completion.parent().type == "instance":
                signature = signature.replace("self, ", "")
        except (AttributeError, TypeError, NotImplementedError):
            signature = completion._get_docstring_signature()
            pass
        return [docstring, signature]