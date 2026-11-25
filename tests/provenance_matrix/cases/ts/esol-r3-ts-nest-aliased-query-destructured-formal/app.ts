import { Controller, Get, Query as SearchQuery } from '@nestjs/common';
@Controller('catalog')
export class CatalogController {
  @Get()
  list(
    @SearchQuery() { label: keyword }: { label?: string },
  ) {
    return keyword;
  }
}
