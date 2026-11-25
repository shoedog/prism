import { Controller, Get, Req } from "@nestjs/common";
import type { Request } from "express";
@Controller()
class AppController {
  @Get()
  find(@Req() req: Request) {
    const value = req.cookies.session;
    return value;
  }
}
