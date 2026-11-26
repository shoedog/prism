function shelves() {
  return [];
}

module.exports = {
  name: 'catalog',
  register(api) {
    api.route({ method: 'GET', path: '/shelves', handler: shelves });
  },
};
