import { f } from "./library";
import * as ns from "./library";
import "./write";
console.log(/*@bareSite*/f(/*@bare*/7), /*@memberSite*/ns.f(/*@member*/7));
