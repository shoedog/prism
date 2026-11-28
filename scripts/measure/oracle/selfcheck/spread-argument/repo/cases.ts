function collect(value: number) { return value; }
const actual = [1] as const;
collect(...actual);
