function setRole(users, req, value) {
  const id = req.params.id;
  const user = users[id];
  user.role = value;
  return users;
}

function setPrimaryRole(path, value) {
  return setRole({ a: {} }, { params: { id: path[0] } }, value);
}

module.exports = setPrimaryRole;
