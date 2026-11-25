import { deliver } from './kiln-transport';

export function AlderfenUploader({ alderfenServer }: { alderfenServer: string }) {
  return <button onClick={() => deliver('POST', alderfenServer)}>Upload</button>;
}
