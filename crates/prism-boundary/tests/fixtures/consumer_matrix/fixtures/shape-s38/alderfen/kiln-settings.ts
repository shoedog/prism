type Deployment = { mount?: string; relay?: string; outlet?: string };
const deployment = (globalThis as { alderfenDeployment?: Deployment }).alderfenDeployment;
export const alderfenOrigin = 'https://alderfen.invalid';
export const alderfenMount = deployment?.mount ?? '/';
export const alderfenPublicStem = deployment?.mount ?? '';
export const alderfenRelay = deployment?.relay ?? alderfenOrigin;
export const alderfenBadge = 'clay';
export const alderfenOutlet = deployment?.outlet ?? alderfenOrigin + '/sensors';
