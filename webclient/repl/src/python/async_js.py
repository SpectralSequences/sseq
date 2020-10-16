from asyncio import AbstractEventLoop
from functools import partial 
from js import sleep
from js import fetch as jsfetch
import json

class WebLoop(AbstractEventLoop):
    def call_soon(self, coro):
        self.step(coro)

    def step(self, coro, arg=None):
        try:
            x = coro.send(arg)
            x = x.then(partial(self.step, coro))
            x.catch(partial(self.fail,coro))
        except StopIteration as result:
            pass

    def fail(self, coro,arg=None):
        try:
            coro.throw(PromiseException(arg))
        except StopIteration:
            pass


class Waiter:
    def __init__(self, time):
        self.time = time
    def __await__(self):
        promise = sleep(self.time)
        yield promise
        return self.time

class Request:
    def __init__(self, promise):
        self.promise = promise
        self._response = None

    def __await__(self):
        _resp = yield self.promise
        return Response(_resp)

    async def __aenter__(self):
        return await self

    async def __aexit__(self, exc_type, exc_value, traceback):
        pass

    async def json(self):
        result = await self.text()
        return json.loads(result)

    async def text(self):
        if not self._response:
            await self
        result = await wrap_promise(self._response.text())
        return result

class Response:
    def __init__(self, resp):
        self._response = resp

    @property
    def ok(self):
        return self._response.ok
    
    @property
    def status(self):
        return int(self._response.status)

    @property
    def status_text(self):
        return self._response.statusText

    async def json(self):
        result = await self.text()
        return json.loads(result)

    async def text(self):
        result = await wrap_promise(self._response.text())
        return result
    async def read_chunks(self):
        stream_reader = self._response.body.getReader()

        while 1:
            chunk = await wrap_promise(stream_reader.read())
            if chunk['value'] is not None:
                yield chunk['value']
            if chunk['done']:
                return

    def __repr__(self):
        return f"Response(status={self.status}, status_text='{self.status_text}')"


class WrappedPromise:
    def __init__(self, promise):
        self.promise = promise
    def __await__(self):
        x = yield self.promise
        return x


def wrap_promise(promise):
    return WrappedPromise(promise)


class Fetcher:
    def __init__(self, base_url=""):
        self.base_url = base_url

    def get(self, path):
        promise = jsfetch(self.base_url+path, dict(
            method="GET"
        ))
        return Request(promise)

    def put(self, path, body):
        promise = jsfetch(self.base_url+path, dict(
            method= "PUT",
            headers= {
                'Accept': 'application/json',
                'Content-Type': 'application/json'
            },
            body= json.dumps(body)
        ))
        return Request(promise)

def sleep(time):
    return Waiter(time)

class PromiseException(RuntimeError):
    pass