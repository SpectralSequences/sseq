import { IndexedDBStorage } from './indexedDB';

export class History {
    constructor() {
        this.store = new IndexedDBStorage('sseq-repl-history', 2);
        this.databaseReady = this.openDatabase();
        this.historyStrings = [];
        this.temporaryValues = [];
        this.stringsFromStorage = [];
        this.undoStack = [];
        this.redoStack = [];
        window.addEventListener('pagehide', this.stowHistory.bind(this));
    }

    async getItem(key) {
        await this.databaseReady;
        if (key in this.temporaryValues) {
            return this.temporaryValues[key];
        }
        if (key in this.historyStrings) {
            return this.historyStrings[key];
        }
        if (key in this.stringsFromStorage) {
            return this.stringsFromStorage[key];
        }
        await this.store.open();
        this.stringsFromStorage[key] = await this.store
            .readTransaction()
            .getItem(key);
        return this.stringsFromStorage[key];
    }

    async openDatabase() {
        await this.store.open();
        await this.commitStowedHistories();
        this.storedHistoryLength =
            (await this.store.readTransaction().getItem('length')) || 0;
        this.idx = this.storedHistoryLength;
    }

    get length() {
        return this.historyStrings.length || this.storedHistoryLength;
    }

    async push(value) {
        value = value.trim();
        await this.databaseReady;
        let mostRecentValue = await this.getItem(this.length - 1);
        // If we use the same command twice, don't add another to history.
        if (value !== mostRecentValue) {
            this.historyStrings[this.length] = value;
        }
        this.idx = this.length;
        this.temporaryValues = [];
        this.undoStack = [];
        this.redoStack = [];
    }

    step(didx) {
        let oldIdx = this.idx;
        this.idx = Math.min(Math.max(this.idx + didx, 0), this.length);
        if (this.idx === oldIdx) {
            return false;
        }
        this.redoStack = [];
        this.undoStack.push(didx);
        return true;
    }

    get value() {
        return this.getItem(this.idx);
    }

    setTemporaryValue(value) {
        this.temporaryValues[this.idx] = value;
    }

    undoStep() {
        let didx = this.undoStack.pop();
        this.idx -= didx;
        this.redoStack.push(didx);
    }

    redoStep() {
        let didx = this.redoStack.pop();
        this.historyIdx += didx;
        this.undoStack.push(didx);
    }

    static get stowagePrefix() {
        return 'history-stowage-';
    }

    stowHistory() {
        let toStow = [];
        this.historyStrings.forEach(e => toStow.push(e));
        if (toStow.length === 0) {
            return;
        }
        localStorage.setItem(
            `${History.stowagePrefix}${Date.now()}`,
            JSON.stringify(toStow),
        );
    }

    async commitStowedHistories() {
        // Don't await this.databaseReady; we are getting called inside of openDatabase
        // so that would deadlock.
        let keysToStore = [];
        for (let i = 0; i < localStorage.length; i++) {
            let key = localStorage.key(i);
            if (key.startsWith(History.stowagePrefix)) {
                keysToStore.push(
                    Number(key.slice(History.stowagePrefix.length)),
                );
            }
        }
        keysToStore.sort((a, b) => a - b);
        if (keysToStore.length === 0) {
            return;
        }
        let transaction = this.store.writeTransaction();
        let lastCommitTime = (await transaction.getItem('lastCommitTime')) || 0;
        let length = (await transaction.getItem('length')) || 0;
        let requests = [];
        keysToStore = keysToStore.filter(v => v > lastCommitTime);
        for (let key of keysToStore) {
            let localStorageKey = `${History.stowagePrefix}${key}`;
            let stowedHistory = JSON.parse(
                localStorage.getItem(localStorageKey),
            );
            // console.log(stowedHistory);
            localStorage.removeItem(localStorageKey);
            for (let histItem of stowedHistory) {
                requests.push(transaction.setItem(length, histItem));
                length++;
            }
        }
        let newLastCommitTime = keysToStore[keysToStore.length - 1];
        requests.push(transaction.setItem('length', length));
        requests.push(transaction.setItem('lastCommitTime', newLastCommitTime));
        await Promise.all(requests);
    }
}

window.History = History;
