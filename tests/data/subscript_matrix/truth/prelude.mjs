function src() {
    return "TAINT-7f3a";
}

function sink(value) {
    const pending = [value];
    const seen = new Set();
    while (pending.length) {
        const item = pending.pop();
        if (typeof item === "string" && item.includes(src())) {
            console.log("OBSERVE reaches");
            return;
        }
        if (item === null || (typeof item !== "object" && typeof item !== "function")) {
            continue;
        }
        if (seen.has(item)) continue;
        seen.add(item);
        if (item instanceof Map) {
            for (const [key, entry] of Map.prototype.entries.call(item)) {
                pending.push(key, entry);
            }
        }
        if (item instanceof Set) {
            for (const entry of Set.prototype.values.call(item)) pending.push(entry);
        }
        for (const key of Reflect.ownKeys(item)) {
            pending.push(key, Reflect.get(item, key));
        }
    }
    console.log("OBSERVE not");
}
