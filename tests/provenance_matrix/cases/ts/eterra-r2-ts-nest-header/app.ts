import { Controller, Get, Headers } from "@nestjs/common";
@Controller("items")
class ItemsController {
  @Get()
  find(@Headers("authorization") token: string) {
    return token;
  }
}
