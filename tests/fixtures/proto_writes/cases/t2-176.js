function createTicket(scope, field = "state") {
  const defaults = { state: "pending" };
  const preview = Object.create(defaults);
  const scopes = Object.assign(Object.create(null), { team: defaults, preview });
  function Ticket(key) {
    this[key] = "open";
  }
  const Factory = Ticket.bind(scopes[scope], field);
  return new Factory();
}

module.exports = createTicket;
