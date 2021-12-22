export function openSocket(initialData, onMessage) {
    const webSocket = new WebSocket(`ws://${window.location.host}/ws`);

    webSocket.onopen = () => {
        for (const data of initialData) {
            window.send(data);
        }
    };

    webSocket.onmessage = onMessage;

    return msg => webSocket.send(JSON.stringify(msg));
}
