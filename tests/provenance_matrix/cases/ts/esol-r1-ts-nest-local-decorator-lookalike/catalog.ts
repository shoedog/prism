import { Controller, Get } from '@nestjs/common';

function Query(_name: string): ParameterDecorator {
  return () => {};
}

@Controller('catalog')
export class Catalog {
  @Get()
  list(@Query('label') label: string) {
    return label;
  }
}
