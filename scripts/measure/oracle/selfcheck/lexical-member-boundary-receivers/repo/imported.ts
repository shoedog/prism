import { receiver, reset, /*@aliasPlainDef*/cell } from './exported';
receiver./*@importWrite*/m = 1;
reset();
export const value = receiver./*@importRead*/m;
export const plain = /*@aliasPlainUse*/cell;
