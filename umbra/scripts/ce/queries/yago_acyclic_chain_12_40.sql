select count(*) from yago2 yago2_0, yago2 yago2_1, yago47, yago29, yago36 yago36_4, yago17, yago13 yago13_6, yago22 yago22_7, yago13 yago13_8, yago13 yago13_9, yago22 yago22_10, yago36 yago36_11 where yago2_0.s = yago2_1.s and yago2_1.d = yago17.d and yago47.s = yago29.d and yago47.d = yago36_11.d and yago29.s = yago36_4.d and yago17.s = yago13_6.d and yago13_6.s = yago22_7.s and yago22_7.d = yago13_8.d and yago13_8.s = yago13_9.s and yago13_9.d = yago22_10.d and yago22_10.s = yago36_11.s;