import { Controller, Get, Query } from '@nestjs/common';
@Controller()
class SearchController {
  @Get()
  find(@Query('q') term: string) {
    term = 'fixed';
    return term;
  }
}
