import { Controller, Get, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

class BaseReports {
  @Get('summary')
  summary() {
    return [];
  }

  @Get('draft')
  draft() {
    return [];
  }
}

@Controller('reports')
class ReportsController extends BaseReports {
  draft() {
    return ['internal'];
  }
}

@Module({ controllers: [ReportsController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
