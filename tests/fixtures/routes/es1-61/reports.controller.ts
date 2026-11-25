import { Controller, Get } from '@nestjs/common';

@Controller('reports')
export class ReportsController {
  @Get('summary')
  summary() {
    return [];
  }
}

@Controller('drafts')
export class DraftsController {
  @Get('pending')
  pending() {
    return [];
  }
}
