def temp():
    def reset_global_handlers():
        global HANDLERS
        HANDLERS = {}
    reset_global_handlers()

    def collect_handlers(field_name):
        def helper(cls):
            setattr(cls, field_name, HANDLERS)
            reset_global_handlers()
            return cls
        return helper
    
    def handle(cmd):
        def helper(func):
            HANDLERS[cmd] = func
            return func
        return helper
    return [collect_handlers, handle]

[collect_handlers, handle] = temp()
del temp