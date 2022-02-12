import * as IDBKeyVal from 'idb-keyval';

export class History {
    constructor() {
        this.store = IDBKeyVal.createStore(
            'sseq-repl-history2',
            'sseq-repl-history2',
        );
        this.databaseReady = this.openDatabase();
        this.historyStrings = [];
        this.temporaryValues = [];
        this.stringsFromStorage = [];
        this.undoStack = [];
        this.redoStack = [];
        this.reverse_search_position;
        window.addEventListener('pagehide', this.stowHistory.bind(this));
    }

    async getItem(key) {
        await this.databaseReady;
        if (key > this.length) {
            return undefined;
        }
        if (key in this.temporaryValues) {
            return this.temporaryValues[key];
        }
        if (key in this.historyStrings) {
            return this.historyStrings[key];
        }
        if (key in this.stringsFromStorage) {
            return this.stringsFromStorage[key];
        }
        await this.fetchRangeFromStorage(key - 10, key + 1);
        return this.stringsFromStorage[key];
    }

    async fetchRangeFromStorage(min, max) {
        max = Math.min(max, this.length);
        min = Math.max(min, 0);
        const keys = Array.from({ length: max - min }, (_, i) => min + i);
        const values = await IDBKeyVal.getMany(keys, this.store);
        for (let i = 0; i < keys.length; i++) {
            this.stringsFromStorage[keys[i]] = values[i];
        }
    }

    async openDatabase() {
        await this.commitStowedHistories();
        this.storedHistoryLength =
            (await IDBKeyVal.get('length', this.store)) || 0;
        if (this.storedHistoryLength > 0) {
            await this.fetchRangeFromStorage(
                this.storedHistoryLength - 10,
                this.storedHistoryLength,
            );
        }
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
        let [lastCommitTime, length] = await IDBKeyVal.getMany(
            ['lastCommitTime', 'length'],
            this.store,
        );
        lastCommitTime = lastCommitTime || 0;
        length = length || 0;
        console.log({ lastCommitTime, length });
        let toSet = [];
        keysToStore = keysToStore.filter(v => v > lastCommitTime);
        for (let key of keysToStore) {
            let localStorageKey = `${History.stowagePrefix}${key}`;
            let stowedHistory = JSON.parse(
                localStorage.getItem(localStorageKey),
            );
            localStorage.removeItem(localStorageKey);
            for (let histItem of stowedHistory) {
                toSet.push([length, histItem]);
                length++;
            }
        }
        let newLastCommitTime = keysToStore[keysToStore.length - 1];
        toSet.push(['length', length]);
        toSet.push(['lastCommitTime', newLastCommitTime]);
        await IDBKeyVal.setMany(toSet, this.store);
    }

    async reverse_history_search(search_str, next = false) {
        const start_idx = this.idx - (next ? 1 : 0);
        for (let i = start_idx; i >= 0; i--) {
            const value = await this.getItem(i);
            if (!value.includes(search_str)) {
                continue;
            }
            // Found
            this.step(i - this.idx);
            return value;
        }
    }

    async forward_history_search(search_str, next = false) {
        const start_idx = this.idx + (next ? 1 : 0);
        const length = this.length;
        for (let i = start_idx; i < length; i++) {
            const value = await this.getItem(i);
            if (!value.includes(search_str)) {
                continue;
            }
            // Found
            this.step(i - this.idx);
            return value;
        }
    }
}

window.History = History;
