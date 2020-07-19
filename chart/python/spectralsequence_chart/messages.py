class Command:
    def set_str(self, cmd_str):
        self.str = cmd_str
        self.filter_list = Command.cmdstr_to_filter_list(self.str)
        self.part_list = self.str.split(".")
        return self
    
    def set_filter_list(self, filter_list):
        self.str = filter_list[0]
        self.filter_list = filter_list
        self.part_list = self.str.split(".")
        return self

    def set_part_list(self, part_list):
        self.str = ".".join(part_list)
        self.filter_list = Command.cmdstr_to_filter_list(self.str)
        self.part_list = part_list
        return self

    def __copy__(self):
        return Command().set_str(self.str)

    @staticmethod
    def cmdstr_to_filter_list(cmd):
        # We use "__" as a standin for "." in "command filter identifiers"
        # Just in case, convert any "__" back to "."
        cmd = cmd.replace("__", ".") # TODO: is this a good choice?
        result = [cmd]
        idx = cmd.rfind(".")
        while( idx >= 0):
            cmd = cmd[ : idx]
            result.append(cmd)
            idx = cmd.rfind(".")
        result.append("*")
        return result

    def __repr__(self):
        return f"""Command("{self.str}")"""

class Message:
    def __init__(self, cmd, args, kwargs):
        # Don't allow top level keys sharing a name with the arguments of handlers.
        for illegal_top_level_key in ["envelope"]:
            if illegal_top_level_key in kwargs:
                raise ValueError(
                    f"""Cannot use key "{illegal_top_level_key}" in top level of message. Ignoring this message:\n""" +\
                    f"""cmd : {cmd}, args : {args}, kwargs : {kwargs}"""
                )
        self.cmd = cmd
        self.args = args
        self.kwargs = kwargs

    def update_arguments(self, **kwargs):
        new_kwargs = self.kwargs.copy()
        new_kwargs.update(kwargs)
        self.kwargs = new_kwargs

    def del_arguments(self, arguments):
        new_kwargs = self.kwargs.copy()
        for argument in arguments:
            del new_kwargs[argument]
        self.kwargs = new_kwargs


    def to_json(self):
        return { "cmd" : self.cmd.filter_list, "args" : self.args, "kwargs" : self.kwargs }

    def __repr__(self):
        return f"""Message(cmd: "{self.cmd.str}", "args": {self.args}, "kwargs": {self.kwargs})"""