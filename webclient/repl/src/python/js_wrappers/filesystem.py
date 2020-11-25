import uuid
from .async_js import wrap_promise
from js import filePicker, requestHandlePermission

class Mode:
    READWRITE = "readwrite"
    READ = "read"

class Permission:
    GRANTED = "granted"
    DENIED = "denied"
    PROMPT = "prompt"

class PickerType:
    DIRECTORY = "directory"
    READWRITE = "readwrite"
    READ = "read"

async def request_handle_permission_a(handle, mode):
    permission = await wrap_promise(handle.queryPermission(dict(mode=mode)))
    if permission == Permission.PROMPT:
        permission = await wrap_promise(requestHandlePermission(handle, mode)  )
    if permission != Permission.GRANTED:
        raise PermissionError("Permission denied.")

class DirectoryHandle:
    def __init__(self):
        self._handle = None

    def _check_handle(self):
        if not self._handle:
            raise RuntimeError("Directory handle not open.")

    def __aiter__(self):
        return DirectoryIterator(self._handle.entries())

    async def open_a(self):
        if self._handle:
            raise RuntimeError("Directory handle is already open")
        response = await wrap_promise(filePicker(PickerType.DIRECTORY))
        self._handle = response[0]

    async def file_handle_a(self, path, create = False):
        self._check_handle()
        permission_needed = Mode.READWRITE if create else Mode.READ
        await request_handle_permission_a(self._handle, permission_needed)
        result = FileHandle()
        result._handle = await wrap_promise(self._handle.getFileHandle(path, dict(create=create)))
        return result

    async def directory_handle_a(self, path, create = False):
        self._check_handle()
        permission_needed = Mode.READWRITE if create else Mode.READ
        await request_handle_permission_a(self._handle, permission_needed)      
        result = DirectoryHandle()
        result._handle = await wrap_promise(self._handle.getDirectoryHandle(path, dict(create=create)))
        return result

    async def remove_entry_a(self, name, recursive = False):
        self._check_handle()
        await self._handle.removeEntry(name, dict(recursive=recursive))

    async def resolve_a(self, handle):
        self._check_handle()
        await wrap_promise(self._handle.resolve(handle._handle))

class DirectoryIterator:
    def __init__(self, iter):
        self._iter = iter
    
    def __aiter__(self):
        return self

    async def __anext__(self):
        e = await wrap_promise(self._iter.next())
        if e["done"]:
            raise StopAsyncIteration
        return e["value"]

class FileHandle:
    def __init__(self):
        self._handle = None

    def is_open(self):
        return self._handle is not None

    def _check_handle(self):
        if not self._handle:
            raise RuntimeError("File handle not open.")

    async def ensure_open_a(self, modify = False):
        if not self.is_open():
            await self.open_a(modify)
        else:
            permission = Mode.READWRITE if modify else Mode.READ
            await request_handle_permission_a(self._handle, permission)

    async def open_a(self, modify = False):
        if self._handle:
            raise RuntimeError("File handle is already open")
        picker_type = PickerType.READWRITE if modify else PickerType.READ
        response = await wrap_promise(filePicker(picker_type))
        self._handle = response[0]

    async def read_text_a(self):
        return await (await self.get_file_a()).text_a()

    async def write_text_a(self, text):
        stream = await self.create_writable_a()
        await stream.write_a(text)
        await stream.close_a()

    async def create_writable_a(self, keep_existing_data = False):
        self._check_handle()
        return WritableFileStream(await wrap_promise(self._handle.createWritable(dict(keepExistingData = keep_existing_data))))

    async def get_file_a(self):
        return File(await wrap_promise(self._handle.getFile()))

class File:
    def __init__(self, file):
        self._file = file

    async def text_a(self):
        return await wrap_promise(self._file.text())

    @property
    def name(self):
        return self._file.name
    
    @property
    def last_modified_date(self):
        return self._file.lastModifiedDate

    #  arrayBuffer, blob


class WritableFileStream:
    def __init__(self, stream):
        self._stream = stream

    async def write_a(self, data, position=None):
        await wrap_promise(self._stream.write(dict(type="write", position=position, data=data)))

    async def seek_a(self, position):
        await wrap_promise(self._stream.seek(position))

    async def truncate_a(self, length):
        await wrap_promise(self._stream.truncate(length))

    async def close_a(self):
        await wrap_promise(self._stream.close())


    

    
