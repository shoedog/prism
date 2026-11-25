import { Controller, Get, Query } from "@nestjs/common";

@Controller("search")
export class Search {
  @Get()
  lookup(@Query("term") term: string) {
    return term;
  }
}
