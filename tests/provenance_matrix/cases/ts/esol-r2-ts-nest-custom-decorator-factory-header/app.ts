import { Controller, Get, createParamDecorator } from '@nestjs/common';
const PreviewMode = createParamDecorator((_data, context) =>
  context.switchToHttp().getRequest().headers['x-preview'],
);
@Controller('catalog')
export class Catalog {
  @Get()
  show(@PreviewMode() preview: string) {
    return preview;
  }
}
