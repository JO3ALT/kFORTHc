program s12;
type
  s4 = array[4] of char;
  one = array[1] of integer;
  rec = record
    a: integer;
    b: integer;
  end;
var
  s: s4;
  o: one;
  r1: rec;
  r2: rec;
begin
  s := 'ABCDE';
  WriteLn(s[0]);
  WriteLn(s[1]);
  WriteLn(s[2]);
  WriteLn(s[3] = #0);

  s := '';
  WriteLn(s[0] = #0);

  o[0] := -2147483647;
  o[0] := o[0] - 1;
  WriteLn(o[0]);
  WriteLn(Length(o));
  WriteLn(Low(o));
  WriteLn(High(o));

  r1.a := 2147483647;
  r1.b := -2147483647;
  r1.b := r1.b - 1;
  r2 := r1;
  WriteLn(r2.a);
  WriteLn(r2.b)
end.
