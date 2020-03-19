global global_handlers
global_handlers = {}
def handler_class(cls):
    global global_handlers
    cls.class_name = cls.__name__
    cls.handlers = global_handlers
    global_handlers = {}
    async def handle_message(self, cmd, data, text):
        await cls.handlers[cmd](self, data, text)
    def has_handler(self, cmd):
        return cmd in cls.handlers
    cls.handle_message = handle_message
    cls.has_handler = has_handler
    return cls

handle_word_len = len("handle")
def handler(func):
    assert(func.__name__.startswith("handle_"))
    global global_handlers
    handler_name = func.__name__[handle_word_len+1:]
    global_handlers[handler_name] = func
    return func

def monkey_patch(cls):
    def helper(func):
        setattr(cls, func.__name__, func)
        return func
    return helper