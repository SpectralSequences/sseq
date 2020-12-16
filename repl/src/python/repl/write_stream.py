class WriteStream:
    """A utility class so we can specify our own handlers for writes to sdout, stderr"""
    def __init__(self, write_handler):
        self.write_handler = write_handler
    
    def write(self, text):
        self.write_handler(text)
