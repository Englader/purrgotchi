import json,sys
from PIL import Image
d=json.load(sys.stdin); S=8; cols=d['cols']; cells=d['cells']; rows=(len(cells)+cols-1)//cols
img=Image.new('RGB',(cols*34*S,rows*34*S),(20,18,28))
for i,c in enumerate(cells):
  ox=(i%cols)*34*S+S; oy=(i//cols)*34*S+S
  for y,row in enumerate(c['px']):
    for x,v in enumerate(row):
      if v:
        col=tuple(int(v[k:k+2],16) for k in (1,3,5))
        for a in range(S):
          for b in range(S): img.putpixel((ox+x*S+a,oy+y*S+b),col)
img.save(sys.argv[1])
