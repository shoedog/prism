import { deliver } from './kiln-transport';

export class AlderfenExporter {
  constructor({ alderfenDestination }: { alderfenDestination: string }) {
    void deliver('POST', alderfenDestination);
  }
}
