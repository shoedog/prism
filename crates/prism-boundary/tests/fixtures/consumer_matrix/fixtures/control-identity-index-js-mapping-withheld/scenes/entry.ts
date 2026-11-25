import renamedQuay from '@quay/quay';
import oldQuay from '@reed-v1/tools/quay';
import explicitQuay from '../net/quay.ts';
import { quay as viaBarrel } from '../net';
import * as shore from '../net';
import { outerQuay } from '../net/bridge';
import board from '../net/board';
import skein from '../net/skein';
import tally from '../net/tally';
import strand from '../net/strand';
import quay from '../net/decoy';

export const rename = () => renamedQuay.get('/cairns'); // GET /cairns
export const oldSpelling = () => oldQuay.get('/cairns'); // GET /cairns
export const relative = () => explicitQuay.get('/cairns'); // GET /cairns
export const barrel = () => viaBarrel.get('/twine'); // GET /twine
export const namespace = () => shore.quay.get('/twine'); // GET /twine
export const bridge = () => outerQuay.get('/twine'); // GET /twine
export const useBoard = () => board.get('/mosaics'); // GET /mosaics
export const useSkein = () => skein.get('/lampwicks'); // GET /lampwicks
export const useTally = () => tally.get('/firs'); // GET /firs
export const useStrand = () => strand.get('/calipers'); // GET /calipers
export const useDecoy = () => quay.get('/cairns');

export function tryLocal(renamedQuay: { get(key: string): number }) {
  return renamedQuay.get('/gates');
}
export function tryBlock() {
  {
    const renamedQuay = quay;
    renamedQuay.get('/gates');
  }
  return renamedQuay.get('/gates'); // GET /gates
}
export function tryDestructured({ quay: renamedQuay }: { quay: typeof quay }) {
  return renamedQuay.get('/gates');
}
