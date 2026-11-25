import {ArgumentsHost, BadRequestException, Catch, Controller, ExceptionFilter, Get, Module, UseFilters} from '@nestjs/common';
import type {Request, Response} from 'express';
@Catch(BadRequestException)
class Report implements ExceptionFilter {
  catch(_error: BadRequestException, host: ArgumentsHost) {
    const http = host.switchToHttp();
    const tag = http.getRequest<Request>().get('x-tag');
    const status = http.getResponse<Response>().statusCode;
    http.getResponse<Response>().json([tag, status]);
  }
}
@Controller('catalog')
@UseFilters(Report)
export class Catalog {
  @Get() show() { throw new BadRequestException(); }
}
@Module({controllers: [Catalog]})
export class AppModule {}
