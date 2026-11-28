interface Array<T> {}
interface Boolean {}
interface CallableFunction {}
interface Function {}
interface IArguments {}
interface NewableFunction {}
interface Number {}
interface Object {}
interface RegExp {}
interface String {}
interface Console { log(...data: any[]): void }
declare var console: Console;
interface Array<T> { [n: number]: T; length: number; }
