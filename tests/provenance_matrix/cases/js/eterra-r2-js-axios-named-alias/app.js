import { get as retrieve } from "axios";
async function run() {
  const response = await retrieve("https://example.org");
  console.log(response);
}
