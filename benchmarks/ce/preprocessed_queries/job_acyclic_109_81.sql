select count(*) from imdb2, imdb122, imdb100, imdb38, imdb14 where imdb2.d = imdb122.d and imdb122.d = imdb100.d and imdb100.d = imdb38.s and imdb38.s = imdb14.s;