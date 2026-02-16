program s02;
type
  rec = record
    x: integer;
    y: integer;
  end;
  iarr = array[4] of integer;
  cube = array[2,2,2] of integer;
var
  r1: rec;
  r2: rec;
  a: iarr;
  b: iarr;
  c: cube;
begin
  r1.x := 10;
  r1.y := 20;
  r2 := r1;
  WriteLn(r2.x);
  WriteLn(r2.y);

  a[0] := 2;
  a[1] := 4;
  a[2] := 6;
  a[3] := 8;
  b := a;
  WriteLn(b[0] + b[3]);

  c[1,1,1] := 77;
  WriteLn(c[1,1,1]);
  WriteLn(Length(a));
  WriteLn(Low(a));
  WriteLn(High(a))
end.
