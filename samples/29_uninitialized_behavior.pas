program s29;
type
  rec = record
    x: integer;
    y: boolean;
    z: char;
  end;
  arr = array[3] of integer;
var
  i: integer;
  b: boolean;
  c: char;
  r: rec;
  a: arr;
begin
  WriteLn(i);
  WriteLn(b);
  WriteLn(Ord(c));
  WriteLn(r.x);
  WriteLn(r.y);
  WriteLn(Ord(r.z));
  WriteLn(a[0]);
  WriteLn(a[1]);
  WriteLn(a[2])
end.
