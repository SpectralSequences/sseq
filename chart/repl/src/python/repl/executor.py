import ast
from asyncio import iscoroutine, get_event_loop
from copy import deepcopy

from .handler_decorator import *
from .completer import Completer
from .execution import Execution


@collect_handlers("message_handlers")
class Executor:
    executor = None
    def __init__(self, send_message_a, namespace=None):
        self.namespace = namespace or {}
        from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
        self.flags = PyCF_ALLOW_TOP_LEVEL_AWAIT
        self.completers = {}
        self.loop = get_event_loop()
        self.send_message_a = send_message_a


    def handle_message(self, cmd, **kwargs):
        if cmd not in self.message_handlers:
            raise Exception(f'Message with unrecognized command "{cmd}"')
        self.loop.call_soon(self.message_handlers[cmd](self, **kwargs))

    @handle("execute")
    async def execute_a(self, **kwargs):
        await Execution(self, **kwargs).run_a()

    @handle("complete")
    async def complete_a(self, uuid, subcmd, **kwargs):
        if subcmd == "new_completer":
            self.completers[uuid] = Completer(self, uuid=uuid, **kwargs)
            return
        if uuid not in self.completers:
            raise Exception(f"No completer with uuid {uuid}")
        await self.completers[uuid].handle_message_a(subcmd=subcmd, **kwargs)
    
    @staticmethod
    def adjust_ast_to_store_result(target_name, tree, code):
        """ Add instruction to store result of expression into a variable with name "target_name"  """
        target = [ast.Name(target_name, ctx = ast.Store())] 
        [tree, result] = Executor.adjust_ast_to_store_result_helper(tree, code)
        tree.body.append(ast.Assign(target, result))
        ast.fix_missing_locations(tree)
        return tree
    
    @staticmethod
    def adjust_ast_to_store_result_helper(tree, code):
        # If the raw source ends in a semicolon, supress the result.
        if code.strip()[-1] == ";":
            return [tree, ast.Constant(None, None)]

        # We directly wrap Expr or Await node in an Assign node.
        last_node = tree.body[-1]
        if isinstance(last_node, (ast.Expr, ast.Await)):
            tree.body.pop()
            return [tree, last_node.value]

        # If node is already an Assign, deep copy the lvalue of the Assign and store that structure
        # into our result.
        # This has the consequence that "[a, b] = (1,2)" returns "[1, 2]", while "a, b = (1,2)" returns "(1,2)".
        # This could be mildly unexpected behavior but it seems entirely harmless.
        # Also in case of l[5] = 7 evaluates l[5] at the end. Python lvalues can be pretty complicated.
        if isinstance(last_node, ast.Assign):
            target = last_node.targets[0] # unclear if targets ever has length greater than 1?
            expr = deepcopy(target)
            # The deep copied expression was an lvalue but we are trying to use it as an rvalue.
            # Need to replace all the "Store" lvalue context markers with "Load" rvalue context markers.
            for x in ast.walk(expr):
                if hasattr(x, "ctx"):
                    x.ctx = ast.Load()
            return [tree, expr]
        # Remaining ast Nodes have no return value (not sure what other possibilities there are actually...)
        return [tree, ast.Constant(None, None)]


    async def run_a(self, code, file):
        mod = ast.parse(code)
        return await self.run_ast_a(code, mod, file)

    async def run_ast_a(self, code, mod, file):
        if len(mod.body) == 0:
            return None
        # The string chosen is not a valid identifier to minimize chances of accidental collision with a user's variables.
        # Collision can only happen if they explicitly write to globals(), definitely not accidental...
        result_target = "EXEC-LAST-EXPRESSION"
        # we need to hand in the source string (self.code) just to check if it ends in a semicolon
        mod = Executor.adjust_ast_to_store_result(result_target, mod, code)        
        flags = self.flags
        ns = self.namespace
        res = eval(compile(mod, file, mode='exec', flags=flags), ns, ns)
        if iscoroutine(res):
            await res
        return ns.pop(result_target)