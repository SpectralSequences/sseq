from asyncio import AbstractEventLoop
from functools import partial 
from js import sleep, fetch as jsfetch
from js.Object import fromEntries as to_js_object
from pyodide import to_js as _to_js
import json

def to_js(arg):
    return _to_js(arg, dict_converter=to_js_object)

class Fetcher:
    def __init__(self, base_url=""):
        self.base_url = base_url

    async def get(self, path):
        return await jsfetch(self.base_url+path, to_js(dict(
            method="GET"
        )))

    async def put(self, path, body):
        return await jsfetch(self.base_url+path, to_js(dict(
            method= "PUT",
            headers= {
                'Accept': 'application/json',
                'Content-Type': 'application/json'
            },
            body= json.dumps(body)
        )))

