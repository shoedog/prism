function set(users, path, v) {
  const [id, field] = path;
  const u = users[id];
  u[field] = v;
  return users;
}

module.exports = set;
