export function openSocket(initialData, onMessage) {
    const resolutionWorker = new Worker('./resolution_worker.js');
    const sseqWorker = new Worker('./sseq_worker.js');

    resolutionWorker.addEventListener('message', ev =>
        sseqWorker.postMessage(ev.data),
    );

    sseqWorker.addEventListener('message', onMessage);

    if (initialData[0].action['Construct']) {
        const name = initialData[0].action['Construct'].module_name;
        const algebra = initialData[0].action['Construct'].algebra_name;

        fetch(`steenrod_modules/${name}.json`).then(response => response.json()).then(json => {
            initialData[0].action = {
                ConstructJson: {
                    algebra_name: algebra,
                    data: JSON.stringify(json),
                },
            };

            for (const data of initialData) {
                send(data);
            }
        })
    } else {
        // Wait for window.send to be defined first
        setTimeout(() => {
            for (const data of initialData) {
                send(data);
            }
        }, 0);
    }

    return msg => {
        const str = JSON.stringify(msg);
        for (const recipient of msg.recipients) {
            if (recipient == 'Sseq') {
                sseqWorker.postMessage(str);
            } else {
                resolutionWorker.postMessage(str);
            }
        }
    };
}
