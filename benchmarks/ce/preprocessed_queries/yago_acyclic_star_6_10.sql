select count(*) from yago36, yago35, yago46_2, yago46_3, yago17, yago46_5 where yago36.s = yago35.s and yago35.s = yago46_2.s and yago46_2.s = yago46_3.s and yago46_3.s = yago17.s and yago17.s = yago46_5.d;