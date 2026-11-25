import { alderfenOrigin, alderfenMount } from 'alderfen/kiln-settings';
const alderfenStem = alderfenMount === '/' ? '' : alderfenMount;
export const AlderfenLink = ({ alderfenMark }: { alderfenMark: string }) =>
  <a href={`${alderfenOrigin}${alderfenStem}/mothbank/brochures/${alderfenMark}`} download>Brochure</a>; // Browser GET /mothbank/brochures/{} on activation
