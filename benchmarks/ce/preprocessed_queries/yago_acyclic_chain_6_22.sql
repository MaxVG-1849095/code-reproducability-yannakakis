select count(*) from yago8, yago25, yago2_2, yago2_3, yago6 where yago8.d = yago25.d and yago25.s = yago2_2.d and yago2_2.s = yago2_3.s and yago2_3.d = yago6.s;