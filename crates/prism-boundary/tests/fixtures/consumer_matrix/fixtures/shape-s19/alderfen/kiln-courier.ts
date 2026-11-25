import { mintAlderfenCourier } from '@alderfen-kit/kiln-transport';
import { alderfenOrigin, alderfenMount } from 'alderfen/kiln-settings';
const alderfenStem = alderfenMount === '/' ? '' : alderfenMount;
export const alderfenCourier = mintAlderfenCourier.create({
  baseURL: `${alderfenOrigin.replace(/\/$/, '')}${alderfenStem}/mothbank/`,
});
