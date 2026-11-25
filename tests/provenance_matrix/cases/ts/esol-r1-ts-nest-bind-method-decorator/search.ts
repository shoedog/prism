import { Bind, Controller, Get, Headers, Query } from '@nestjs/common';

@Controller('catalog')
export class Search {
  @Get()
  @Bind(Query('term'), Headers('x-preview'))
  list(term: string, preview: string) {
    return [term, preview];
  }
}
