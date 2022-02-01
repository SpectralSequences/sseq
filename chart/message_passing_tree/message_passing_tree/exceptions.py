class DoubleFaultError(RuntimeError):
    pass


class BadMessageError(TypeError):
    pass


class MessageMissingCommandError(BadMessageError):
    def __init__(self, data):
        super().__init__(f"""Client sent message missing "cmd" key.""")


class MessageMissingArgumentsError(BadMessageError):
    def __init__(self, key, data):
        super().__init__(f"""Client sent message missing "{key}" key.""")
