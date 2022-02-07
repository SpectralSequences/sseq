from js import filePicker, requestHandlePermission, sleep as sleep_a
from repl.util import to_js


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
    print("query permission", handle.name, mode)
    permission = await handle.queryPermission(dict(mode=mode))
    print(" -- permission:", permission)
    if permission == Permission.PROMPT:
        permission = await requestHandlePermission(handle, mode)
    if permission != Permission.GRANTED:
        raise PermissionError("Permission denied.")


class NotFoundException(IOError):
    pass


class TypeMismatchException(IOError):
    pass


class UserAbortException(IOError):
    pass


def classify_error(e):
    if str(e).find("NotFoundError:") >= 0:
        return NotFoundException
    if str(e).find("TypeMismatchError:") >= 0:
        return TypeMismatchException
    if str(e).find("AbortError:") >= 0:
        return UserAbortException
    return None


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
        err = None
        response = None
        try:
            response = await filePicker(PickerType.DIRECTORY)
        except Exception as e:
            err = classify_error(e)
            if not err:
                raise
        if err:
            raise UserAbortException("User aborted the file picker")
        if not response:
            raise AssertionError()
        self._handle = response[0]

    def is_open(self):
        return self._handle is not None

    def path(self, path):
        return RelativePath(self, path)

    async def file_handle_a(self, path, create=False):
        print("file_handle_a:", path)
        self._check_handle()
        permission_needed = Mode.READWRITE if create else Mode.READ
        print("request permission?", permission_needed)
        await request_handle_permission_a(self._handle, permission_needed)
        print("done requesting permission")
        result = FileHandle()
        err = None
        try:
            print("await...")
            result._handle = await self._handle.getFileHandle(path, create=create)
            print("got result?")
        except Exception as e:
            err = classify_error(e)
            if not err:
                raise
        if err is NotFoundException:
            raise NotFoundException(f"File '{path}' not found")
        if err is TypeMismatchException:
            raise TypeMismatchException("'{path}' is a directory not a file")
        if not result._handle:
            raise AssertionError()
        return result

    async def directory_handle_a(self, path, create=False):
        self._check_handle()
        permission_needed = Mode.READWRITE if create else Mode.READ
        await request_handle_permission_a(self._handle, permission_needed)
        result = DirectoryHandle()
        err = None
        try:
            result._handle = await self._handle.getDirectoryHandle(
                path, dict(create=create)
            )
        except Exception as e:
            err = classify_error(e)
            if not err:
                raise
        if err is NotFoundException:
            raise NotFoundException(f"Directory '{path}' not found")
        if err is TypeMismatchException:
            raise TypeMismatchException("'{path}' is a file not a directory")
        if not result._handle:
            raise AssertionError()
        return result

    async def remove_entry_a(self, name, recursive=False):
        self._check_handle()
        await self._handle.removeEntry(name, dict(recursive=recursive))

    async def resolve_a(self, handle):
        self._check_handle()
        await self._handle.resolve(handle._handle)


class RelativePath:
    def __init__(self, directory_handle, path):
        self.root_handle = directory_handle
        self.path = path.split("/")
        self._target_handle = None

    @staticmethod
    async def follow_dir_path_if_exists_a(handle, path):
        for p in path:
            try:
                handle = await handle.directory_handle_a(p)
            except (TypeMismatchException, NotFoundException):
                return None
        return handle

    @staticmethod
    async def follow_path_if_exists_a(handle, path):
        print("follow_path_if_exists_a:", handle._handle.name, path)
        handle = await RelativePath.follow_dir_path_if_exists_a(handle, path[:-1])
        print(" == ", handle._handle.name, path)
        if not handle:
            return None
        try:
            print("try")
            target_handle = await handle.file_handle_a(path[-1])
        except NotFoundException:
            print(" == not found")
            return None
        except TypeMismatchException:
            print(" == type mismatch")
            await sleep_a(100)
            target_handle = await handle.directory_handle_a(path[-1])
        return target_handle

    @staticmethod
    async def ensure_directory_exists_a(handle, path):
        for p in path:
            handle = await handle.directory_handle_a(p, create=True)
        return handle

    async def exists_a(self):
        if self._target_handle:
            return True
        self._target_handle = await RelativePath.follow_path_if_exists_a(
            self.root_handle, self.path
        )
        return self._target_handle is not None

    async def resolve_file_handle_a(self, create=True, recursive=False):
        if not self._target_handle:
            path = self.path
            handle = self.root_handle
            if recursive:
                directory = await RelativePath.ensure_directory_exists_a(
                    handle, path[:-1]
                )
            else:
                directory = await RelativePath.follow_dir_path_if_exists_a(
                    handle, path[:-1]
                )
            if not directory:
                raise NotFoundException(
                    f"The directory '{'/'.join(path[:-1])} does not exist."
                )
            self._target_handle = await directory.file_handle_a(self.path[-1], create)
        return self._target_handle

    async def write_text_a(self, text):
        handle = await self.resolve_file_handle_a()
        await handle.write_text_a(text)

    async def read_text_a(self):
        if not self._target_handle:
            directory = await RelativePath.follow_dir_path_if_exists_a(
                self.root_handle, self.path[:-1]
            )
            self._target_handle = await directory.file_handle_a(self.path[-1])
        return await self._target_handle.read_text_a()


class DirectoryIterator:
    def __init__(self, iter):
        self._iter = iter

    def __aiter__(self):
        return self

    async def __anext__(self):
        e = await self._iter.next()
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

    async def ensure_open_a(self, modify=False):
        if not self.is_open():
            await self.open_a(modify)
        else:
            permission = Mode.READWRITE if modify else Mode.READ
            await request_handle_permission_a(self._handle, permission)

    async def open_a(self, modify=False):
        if self._handle:
            raise RuntimeError("File handle is already open")
        picker_type = PickerType.READWRITE if modify else PickerType.READ
        response = None
        err = None
        try:
            response = await filePicker(picker_type)
        except Exception as e:
            err = classify_error(e)
            if not err:
                raise
        if err:
            raise UserAbortException("User aborted the file picker")
        if not response:
            raise AssertionError()
        self._handle = response[0]

    async def read_text_a(self):
        return await (await self.get_file_a()).text_a()

    async def write_text_a(self, text):
        stream = await self.create_writable_a()
        await stream.write_a(text)
        await stream.close_a()

    async def create_writable_a(self, keep_existing_data=False):
        self._check_handle()
        return WritableFileStream(
            await self._handle.createWritable(dict(keepExistingData=keep_existing_data))
        )

    async def get_file_a(self):
        return File(await self._handle.getFile())


class File:
    def __init__(self, file):
        self._file = file

    async def text_a(self):
        return await self._file.text()

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
        await self._stream.write(to_js(dict(type="write", position=position, data=data)))

    async def seek_a(self, position):
        await self._stream.seek(position)

    async def truncate_a(self, length):
        await self._stream.truncate(length)

    async def close_a(self):
        await self._stream.close()
