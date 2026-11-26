import quay from './quay';
const stem = '/reels';
const mast = '/spools';
const withQuery = '/top?order=';
export function paths(mark: number, seal: number, query: string, runtimeBase: string) {
  const last = `reels`;
  const direct = '/drums';
  quay.get(direct); // GET /drums
  quay.get(`${mast}/${last}`); // GET /spools/reels
  quay.get(`${stem}?swatch=${query}`); // GET /reels
  quay.get(`${withQuery}${query}`); // GET /top
  quay.get(`/reels/${mark}/ends/${seal}`); // GET /reels/{}/ends/{}
  quay.get('/reels/' + mark); // GET /reels/{}
  quay.get('/reels/' + (mark + 1) + '/ends'); // GET /reels/{}/ends
  quay.get('/spools/' + last + '/' + mark); // GET /spools/reels/{}
  quay.get(`/reels/${mark}?swatch=${query}`); // GET /reels/{}
  quay.get('/reels?swatch=' + query); // GET /reels
  quay.get(`/reels/cord-${mark}`); // GET dynamic
  quay.get(`${runtimeBase}/reels`); // GET dynamic
  quay.get(`/reels/${mark}${seal}`); // GET dynamic
  quay.get('/reels/' + mark + '-cord'); // GET dynamic
  quay.get('/charms/%2F'); // GET /charms/%2F
}
export function shadow(stem: string) {
  return quay.get(stem); // GET dynamic
}
export function weave() {
  const ivory = '/spools/ivory';
  return () => quay.get(ivory); // GET /spools/ivory
}
export const ahead = () => quay.get(nextTrail); // GET /spools/ivory
const nextTrail = '/spools/ivory';
