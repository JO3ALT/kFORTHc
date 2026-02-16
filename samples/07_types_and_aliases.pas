program s07;
type
  i32 = integer;
  flag_t = boolean;
  ch_t = char;
  vec5 = array[5] of integer;
  s8 = array[8] of char;
var
  n: i32;
  f: flag_t;
  ch: ch_t;
  a: vec5;
  s: s8;
begin
  n := 123;
  a[0] := n;
  a[4] := a[0] - 23;
  f := a[4] >= 100;
  ch := 'K';
  s := 'TYPE';

  WriteLn(a[0]);
  WriteLn(a[4]);
  WriteLn(f);
  WriteLn(ch);
  WriteLn(Length(a));
  WriteLn(Low(a));
  WriteLn(High(a));
  WriteStr(s);
  WriteLn;
  WriteLn(s[4] = #0)
end.
