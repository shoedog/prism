import { AlderfenUploader } from '@alderfen-kit/kiln-upload';
import { alderfenOrigin } from 'alderfen/kiln-settings';
export const AlderfenShelf = (alderfenPick: { alderfenBin: string; alderfenMark: string; alderfenStamp: string }) =>
  <AlderfenUploader alderfenServer={`${alderfenOrigin}/mothbank/${alderfenPick.alderfenBin}/${alderfenPick.alderfenMark}/${alderfenPick.alderfenStamp}`} />; // Uploader POST /mothbank/{}/{}/{}
