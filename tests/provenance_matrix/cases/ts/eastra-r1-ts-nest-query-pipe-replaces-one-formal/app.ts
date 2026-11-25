import { Controller, Get, Module, PipeTransform, Query } from '@nestjs/common';
class Fixed implements PipeTransform {
  transform(_value: unknown) { return 'preview'; }
}
@Controller()
class Editions {
  @Get('edition')
  show(@Query('mode', new Fixed()) normalized: string, @Query('region') region: string) {
    return [normalized, region];
  }
}
@Module({ controllers: [Editions] })
export class AppModule {}
