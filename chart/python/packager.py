#!/usr/bin/env python3

PACKAGE = "spectralsequence_chart"
PYTHON_VERSION = "3.8"

TEMPLATE = '''
var Module = typeof pyodide._module !== 'undefined' ? pyodide._module : {{}};

Module.checkABI(1);

if (!Module.expectedDataFileDownloads) {{
  Module.expectedDataFileDownloads = 0;
  Module.finishedDataFileDownloads = 0;
}}
Module.expectedDataFileDownloads++;
(function() {{
 var loadPackage = function(metadata) {{

    var PACKAGE_PATH;
    if (typeof window === 'object') {{
      PACKAGE_PATH = window['encodeURIComponent'](window.location.pathname.toString().substring(0, window.location.pathname.toString().lastIndexOf('/')) + '/');
    }} else if (typeof location !== 'undefined') {{
      // worker
      PACKAGE_PATH = encodeURIComponent(location.pathname.toString().substring(0, location.pathname.toString().lastIndexOf('/')) + '/');
    }} else {{
      throw 'using preloaded data can only be done on a web page or in a web worker';
    }}
    var PACKAGE_NAME = '{package}.data';
    var REMOTE_PACKAGE_BASE = '{package}.data';
    if (typeof Module['locateFilePackage'] === 'function' && !Module['locateFile']) {{
      Module['locateFile'] = Module['locateFilePackage'];
      err('warning: you defined Module.locateFilePackage, that has been renamed to Module.locateFile (using your locateFilePackage for now)');
    }}
    var REMOTE_PACKAGE_NAME = Module['locateFile'] ? Module['locateFile'](REMOTE_PACKAGE_BASE, '') : REMOTE_PACKAGE_BASE;

    var REMOTE_PACKAGE_SIZE = metadata.remote_package_size;
    var PACKAGE_UUID = metadata.package_uuid;

    function fetchRemotePackage(packageName, packageSize, callback, errback) {{
      var xhr = new XMLHttpRequest();
      xhr.open('GET', packageName, true);
      xhr.responseType = 'arraybuffer';
      xhr.onprogress = function(event) {{
        var url = packageName;
        var size = packageSize;
        if (event.total) size = event.total;
        if (event.loaded) {{
          if (!xhr.addedTotal) {{
            xhr.addedTotal = true;
            if (!Module.dataFileDownloads) Module.dataFileDownloads = {{}};
            Module.dataFileDownloads[url] = {{
              loaded: event.loaded,
              total: size
            }};
          }} else {{
            Module.dataFileDownloads[url].loaded = event.loaded;
          }}
          var total = 0;
          var loaded = 0;
          var num = 0;
          for (var download in Module.dataFileDownloads) {{
          var data = Module.dataFileDownloads[download];
            total += data.total;
            loaded += data.loaded;
            num++;
          }}
          total = Math.ceil(total * Module.expectedDataFileDownloads/num);
          if (Module['setStatus']) Module['setStatus']('Downloading data... (' + loaded + '/' + total + ')');
        }} else if (!Module.dataFileDownloads) {{
          if (Module['setStatus']) Module['setStatus']('Downloading data...');
        }}
      }};
      xhr.onerror = function(event) {{
        throw new Error("NetworkError for: " + packageName);
      }}
      xhr.onload = function(event) {{
        if (xhr.status == 200 || xhr.status == 304 || xhr.status == 206 || (xhr.status == 0 && xhr.response)) {{ // file URLs can return 0
          var packageData = xhr.response;
          callback(packageData);
        }} else {{
          throw new Error(xhr.statusText + " : " + xhr.responseURL);
        }}
      }};
      xhr.send(null);
    }};

    function handleError(error) {{
      console.error('package error:', error);
    }};

      var fetchedCallback = null;
      var fetched = Module['getPreloadedPackage'] ? Module['getPreloadedPackage'](REMOTE_PACKAGE_NAME, REMOTE_PACKAGE_SIZE) : null;

      if (!fetched) fetchRemotePackage(REMOTE_PACKAGE_NAME, REMOTE_PACKAGE_SIZE, function(data) {{
        if (fetchedCallback) {{
          fetchedCallback(data);
          fetchedCallback = null;
        }} else {{
          fetched = data;
        }}
      }}, handleError);

  function runWithFS() {{

    function assert(check, msg) {{
      if (!check) throw msg + new Error().stack;
    }}
    Module['FS_createPath']('/', 'lib', true, true);
    Module['FS_createPath']('/lib', 'python{python_version}', true, true);
    Module['FS_createPath']('/lib/python{python_version}', 'site-packages', true, true);
    Module['FS_createPath']('/lib/python{python_version}/site-packages', '{package}', true, true);
    Module['FS_createPath']('/lib/python{python_version}/site-packages', '{package}.egg-info', true, true);

    function DataRequest(start, end, audio) {{
      this.start = start;
      this.end = end;
      this.audio = audio;
    }}
    DataRequest.prototype = {{
      requests: {{}},
      open: function(mode, name) {{
        this.name = name;
        this.requests[name] = this;
        Module['addRunDependency']('fp ' + this.name);
      }},
      send: function() {{}},
      onload: function() {{
        var byteArray = this.byteArray.subarray(this.start, this.end);
        this.finish(byteArray);
      }},
      finish: function(byteArray) {{
        var that = this;

        Module['FS_createPreloadedFile'](this.name, null, byteArray, true, true, function() {{
          Module['removeRunDependency']('fp ' + that.name);
        }}, function() {{
          if (that.audio) {{
            Module['removeRunDependency']('fp ' + that.name); // workaround for chromium bug 124926 (still no audio with this, but at least we don't hang)
          }} else {{
            err('Preloading file ' + that.name + ' failed');
          }}
        }}, false, true); // canOwn this data in the filesystem, it is a slide into the heap that will never change

        this.requests[this.name] = null;
      }}
    }};

        var files = metadata.files;
        for (var i = 0; i < files.length; ++i) {{
          new DataRequest(files[i].start, files[i].end, files[i].audio).open('GET', files[i].filename);
        }}


    function processPackageData(arrayBuffer) {{
      Module.finishedDataFileDownloads++;
      assert(arrayBuffer, 'Loading data file failed.');
      assert(arrayBuffer instanceof ArrayBuffer, 'bad input to processPackageData');
      var byteArray = new Uint8Array(arrayBuffer);
      var curr;

      // Reuse the bytearray from the XHR as the source for file reads.
      DataRequest.prototype.byteArray = byteArray;

        var files = metadata.files;
        for (var i = 0; i < files.length; ++i) {{
          DataRequest.prototype.requests[files[i].filename].onload();
        }}
            Module['removeRunDependency']('datafile_{package}.data');

    }};
    Module['addRunDependency']('datafile_{package}.data');

    if (!Module.preloadResults) Module.preloadResults = {{}};

      Module.preloadResults[PACKAGE_NAME] = {{fromCache: false}};
      if (fetched) {{
        processPackageData(fetched);
        fetched = null;
      }} else {{
        fetchedCallback = processPackageData;
      }}

  }}
  if (Module['calledRun']) {{
    runWithFS();
  }} else {{
    if (!Module['preRun']) Module['preRun'] = [];
    Module["preRun"].push(runWithFS); // FS is not initialized yet, wait for it
  }}

 }}
 loadPackage({files});

}})();
'''

import pathlib
import json
from uuid import uuid4

files = list(pathlib.Path(PACKAGE).glob("*.py"))
files.extend(pathlib.Path(f"{PACKAGE}.egg-info").glob("*"))
metadata = []
start = 0
data = []
for file in files:
    info = {
        "filename": f"/lib/python{PYTHON_VERSION}/site-packages/{file}",
        "start": start,
        "audio": 0,
    }

    cur = file.read_bytes()
    start += len(cur)
    info['end'] = start
    data.append(cur)
    metadata.append(info)

pathlib.Path(f"{PACKAGE}.data").write_bytes(b"".join(data))

pathlib.Path(f"{PACKAGE}.js").write_text(
    TEMPLATE.format(
        package=PACKAGE,
        python_version=PYTHON_VERSION,
        files=json.dumps({
            "files": metadata,
            "remote_package_size": start,
            "package_uuid" : str(uuid4())
        })
    )
)