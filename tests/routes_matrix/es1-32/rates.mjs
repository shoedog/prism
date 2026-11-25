function rates() { return []; }
export const plugin = {
  name: 'rates',
  register(api) {
    api.route({ method: 'GET', path: '/rates', handler: rates });
  }
};
