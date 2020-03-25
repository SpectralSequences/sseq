global global_handlers
global_handlers = {}
def handler_class(cls):
    """ Decorator to indicate that the class has event handlers.
        Defines the command obj.handle_message(cmd, data) to dispatch into the list of handlers.
        Methods named "handle_command" can be marked with @handler to indicate that they should be added
        to the event dispatch for this class.
    """
    # The handlers in the class body are called first and store the handlers into global_handlers.
    # We now copy these methods into cls.handlers and clear global_handlers for the next handler class.
    global global_handlers 
    cls.handlers = global_handlers
    global_handlers = {}
    
    async def handle_message(self, cmd, data):
        """ Handle a message by dispatch into cls.handler """
        if cmd in self.handlers:
            await self.handlers[cmd](self, data)
        elif cmd in cls.handlers:
            await cls.handlers[cmd](self, data)
        else:
            raise KeyError(f"""Received unknown command "{cmd}" from client. Received with data "${data}".""")
    
    def has_handler(self, cmd):
        return cmd in self.handlers or cmd in cls.handlers
        
    def set_handler(self, cmd, func):
        self.handlers[cmd] = func

    def __new_init__(self, *args, **kwargs):
        self.handlers = {}
        __old_init__(self, args, kwargs)

    cls.set_handler = set_handler
    cls.handle_message = handle_message
    cls.has_handler = has_handler
    cls.__old_init__ = cls.__init__
    cls.__init = __new_init__
    return cls

handle_word_len = len("handle")
def handler(func):
    assert(func.__name__.startswith("handle_"))
    global global_handlers
    handler_name = func.__name__[handle_word_len+1:]
    global_handlers[handler_name] = func
    return func

class Dispatch(dict):
    def __init__(self, prefix, cmd_name):
        self.prefix = prefix
        self.cmd_name = cmd_name

    def handle_event(self, msg):
        if self.cmd_name not in msg:
            raise ValueError("Missing command")
        if msg[self.cmd_name] not in self:
            raise ValueError("Undefined command")
        self[msg[self.cmd_name]](msg)

    def __call__(self, func):
        assert(func.__name__.startswith(self.prefix))
        handler_name = func.__name__[len(self.prefix):]
        self[handler_name] = func
        return func

def monkey_patch(cls):
    def helper(func):
        setattr(cls, func.__name__, func)
        return func
    return helper