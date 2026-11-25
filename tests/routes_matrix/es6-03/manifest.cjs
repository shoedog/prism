module.exports = {
  register: {
    plugins: [{ plugin: './catalog.cjs', routes: { prefix: '/api' } }],
  },
};
