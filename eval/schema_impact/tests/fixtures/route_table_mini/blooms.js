import { pressBloom as chooseBloom } from "./press";

function pickBloom() {}

const spare = [{ verb: "get", pattern: "/ignored", target: pickBloom }];

this.server.register([
  { verb: "get", pattern: "/blooms/:petal", target: pickBloom },
  { verb: "post", pattern: "/blooms", config: { target: chooseBloom, check: {} } },
  { verb: "delete", pattern: "/blooms/{petal}", target: garden.missing },
]);
