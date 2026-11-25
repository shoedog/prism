local mode = 'default'
if enabled then
  mode = os.getenv('MODE')
end
print(mode)
