import { alderfenPublicStem } from 'alderfen/kiln-settings';
const probeAlderfen = (alderfenTrail: string) => fetch(alderfenTrail); // GET dynamic under the deployment mount
export const tallyAlderfen = () => probeAlderfen(`${alderfenPublicStem}/sprig-loader.js`);
