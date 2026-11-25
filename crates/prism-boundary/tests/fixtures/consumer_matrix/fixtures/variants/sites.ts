import quay from './quay';

const current = 'v5';
export function byVersion(version: 'v4' | 'v5') {
  quay.get(`/api/${version}/seals`); // GET /api/{}/seals
  quay.get(`/api/${current}/seals`); // GET /api/v5/seals
}
export const byTenant = (tenant: 'ochre' | 'cobalt') =>
  quay.get(`/tenants/${tenant}/seals`); // GET /tenants/{}/seals
export const byBranch = (stocked: boolean) =>
  quay.get(stocked ? '/basalt' : '/calcite'); // GET /basalt or GET /calcite
export const bySameBranch = (stocked: boolean) =>
  quay.get(stocked ? '/basalt?full=1' : '/basalt?full=0'); // GET /basalt
