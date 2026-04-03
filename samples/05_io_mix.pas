program s05;
type
  iarr = array[4] of integer;
var
  i: integer;
  b: boolean;
  a: iarr;
begin
  Read(i);
  ReadLn;
  Read(b);
  ReadLn;
  Read(a[0], a[1], a[2], a[3]);

  WriteLn(i);
  WriteLn(b);
  WriteLn(a[0]);
  WriteLn(a[1]);
  WriteLn(a[2]);
  WriteLn(a[3])
end.
