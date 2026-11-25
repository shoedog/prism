import { CanActivate, Controller, ExecutionContext, Get, Injectable, Module, UseGuards } from '@nestjs/common';
import type { Request } from 'express';
@Injectable()
class EditionGuard implements CanActivate {
  canActivate(context: ExecutionContext) {
    const language = context.switchToHttp().getRequest<Request>().headers['accept-language'];
    const handlerName = context.getHandler().name;
    void [language, handlerName];
    return true;
  }
}
@Controller()
@UseGuards(EditionGuard)
class Editions {
  @Get('edition') show() { return 'preview'; }
}
@Module({ controllers: [Editions], providers: [EditionGuard] })
export class AppModule {}
