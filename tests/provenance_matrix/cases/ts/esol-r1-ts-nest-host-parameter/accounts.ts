import { Controller, Get, HostParam } from '@nestjs/common';

@Controller({ host: ':tenant.example.test' })
export class Accounts {
  @Get()
  index(
    @HostParam('tenant') accountKey: string,
  ) {
    return accountKey;
  }
}
