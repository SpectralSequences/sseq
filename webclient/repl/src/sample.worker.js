// sample.worker.js
self.addEventListener('message', (event) => {
  console.log("sample worker received message", event);
  if (event.data === 'ping') {
    self.postMessage('pong')
  }
})