import { Controller, Get, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller({ path: 'reports', host: 'staff.example.test' })
class StaffController {
  @Get('summary')
  staff() { return 'staff summary'; }
}

@Controller({ path: 'reports', host: ':tenant.accounts.example.test' })
class TenantController {
  @Get('summary')
  tenant() { return 'tenant summary'; }
}

@Module({ controllers: [StaffController, TenantController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
