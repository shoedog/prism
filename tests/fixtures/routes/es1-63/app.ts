import { Controller, Head as Probe, RequestMapping, RequestMethod, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('reports')
class ReportsController {
  @Probe('daily')
  head() {
    return;
  }

  @RequestMapping({ path: 'daily', method: RequestMethod.OPTIONS })
  options() {
    return;
  }
}

@Module({ controllers: [ReportsController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
