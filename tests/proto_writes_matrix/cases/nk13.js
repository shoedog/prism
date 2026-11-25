const EventType = { Click: "click" };

function put(handlers, phase, v) {
  handlers[EventType.Click][phase] = v;
  return handlers;
}

module.exports = put;
