from .send_message import send_message
from .handler_decorator import *
from uuid import uuid4
from collections import OrderedDict

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

    def handle_message(self, subcmd, **kwargs):
        if subcmd not in self.message_handlers:
            raise Exception(f'Message with unrecognized subcommand "{subcmd}"')
        handler = self.message_handlers[subcmd]
        handler(self, **kwargs)

    def send_message(self, subcmd, subuuid, **kwargs):
        send_message("complete", self.uuid, subcmd=subcmd, subuuid=subuuid, **kwargs)
    
    @handle("completions")
    def get_completions(self, subuuid, code, lineNumber, column):
        import jedi
        self.code = code
        state_id = str(uuid4())
        # print("get completions", code)
        completions = jedi.Interpreter(code, [self.executor.namespace]) \
                        .complete(line=lineNumber, column=column - 1, fuzzy=True)
        # print("got completions", code)
        self.states[state_id] = completions
        result = []
        for comp in completions:
            # docstring = comp.docstring(raw=True)
            result.append(dict(
                name=comp.name, 
                kind=comp.type
            ))
        self.send_message("completions", subuuid, completions=result, state_id=state_id)

    @handle("completion_detail")
    def get_completion_info(self, subuuid, state_id, idx):
        completion = self.states[state_id][idx]
        try:
            if completion.type == "instance":
                [docstring, signature] = self.get_completion_info_instance(subuuid, completion)
            elif completion.type in ["keyword", "module"]:
                docstring = completion.docstring()
                signature = ""
            else:
                [docstring, signature] = self.get_completion_info_standard(subuuid, completion)
        except Exception as e:
            print("Error triggered during completion detail for", completion.name, "type:", completion.type);
            raise
        # import re
        # regex = re.compile('(?<!\n)\n(?!\n)', re.MULTILINE) # Remove isolated newline characters.
        # docstring = regex.sub("", docstring)
        self.send_message("completion_detail", subuuid, docstring=docstring, signature=signature)
        
    
    def get_completion_info_instance(self, subuuid, completion):
        """ Jedi by default does a bad job of getting the completion info for "instances". 
            If the instance is a property on a class with an available docstring, then we report that.
            In any case, give the signature as "name: type".
        """
        docstring = ""
        type_string = ""   
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
        return [docstring, signature]

    def get_completion_info_standard(self, subuuid, completion):
        docstring = completion.docstring(raw=True) or completion._get_docstring()
        try:
            signature = completion.get_type_hint()
            object = completion.get_signatures()[0]._name._value.access_handle.access._obj
            if type(object).__name__ == "method":
                signature = signature.replace("self, ", "")
        except (AttributeError, TypeError, NotImplementedError):
            signature = completion._get_docstring_signature()
            pass
        return [docstring, signature]