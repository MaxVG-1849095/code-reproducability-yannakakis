select count(*) from imdb1, imdb117, imdb2, imdb9 where imdb1.s = imdb117.s and imdb117.d = imdb2.d and imdb2.d = imdb9.s;